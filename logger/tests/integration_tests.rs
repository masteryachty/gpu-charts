use logger::{
    common::{
        data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData},
        AnalyticsEngine, DataBuffer, SymbolMapper,
    },
    config::{AssetGroup, Config, EquivalenceRules, SymbolMappingsConfig},
    exchanges::Message,
};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_symbol_mapper_integration() {
    let config = SymbolMappingsConfig {
        mappings_file: None,
        auto_discover: true,
        equivalence_rules: EquivalenceRules {
            quote_assets: vec![AssetGroup {
                group: "USD_EQUIVALENT".to_string(),
                members: vec!["USD".to_string(), "USDT".to_string()],
                primary: "USD".to_string(),
            }],
        },
    };

    let mapper = SymbolMapper::new(config).unwrap();

    // Add symbols from different exchanges
    mapper.add_symbol(logger::common::data_types::Symbol {
        exchange: ExchangeId::Coinbase,
        exchange_symbol: "BTC-USD".to_string(),
        normalized: "BTC-USD".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        asset_class: logger::common::data_types::AssetClass::Spot,
        active: true,
        min_size: None,
        tick_size: None,
    });

    mapper.add_symbol(logger::common::data_types::Symbol {
        exchange: ExchangeId::Binance,
        exchange_symbol: "BTCUSDT".to_string(),
        normalized: "BTC-USDT".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        asset_class: logger::common::data_types::AssetClass::Spot,
        active: true,
        min_size: None,
        tick_size: None,
    });

    // Test normalization
    assert_eq!(
        mapper.normalize(ExchangeId::Coinbase, "BTC-USD"),
        Some("BTC-USD".to_string())
    );
    assert_eq!(
        mapper.normalize(ExchangeId::Binance, "BTCUSDT"),
        Some("BTC-USDT".to_string())
    );

    // Test finding related symbols
    let btc_usd_pairs = mapper.find_related("BTC", "USD");
    assert_eq!(btc_usd_pairs.len(), 1);
    assert_eq!(btc_usd_pairs[0].symbol, "BTC-USD");
}

#[tokio::test]
async fn test_data_buffer_integration() {
    let temp_dir = TempDir::new().unwrap();
    let buffer = DataBuffer::new(temp_dir.path().to_path_buf());

    // Add market data
    let market_data = UnifiedMarketData {
        exchange: ExchangeId::Coinbase,
        symbol: "BTC-USD".to_string(),
        timestamp: 1672531200,
        nanos: 0,
        price: 50000.0,
        volume: 0.1,
        side: TradeSide::Buy,
        best_bid: 49999.0,
        best_ask: 50001.0,
        exchange_specific: None,
    };

    buffer.add_market_data(market_data).await.unwrap();

    // Add trade data
    let trade_data = UnifiedTradeData {
        exchange: ExchangeId::Binance,
        symbol: "ETH-USDT".to_string(),
        trade_id: 123456,
        timestamp: 1672531200,
        nanos: 0,
        price: 3000.0,
        size: 0.5,
        side: TradeSide::Sell,
        maker_order_id: [0; 16],
        taker_order_id: [0; 16],
        exchange_specific: None,
    };

    buffer.add_trade_data(trade_data).await.unwrap();

    // Flush to disk
    buffer.flush_to_disk().await.unwrap();

    // Verify files were created
    assert!(temp_dir.path().join("coinbase/BTC-USD/MD").exists());
    assert!(temp_dir.path().join("binance/ETH-USDT/TRADES").exists());
}

#[tokio::test]
async fn test_analytics_engine() {
    let engine = AnalyticsEngine::new(10000.0, Duration::from_secs(30));

    // Process multiple trades
    let trades = vec![
        UnifiedTradeData {
            exchange: ExchangeId::Coinbase,
            symbol: "BTC-USD".to_string(),
            trade_id: 1,
            timestamp: 1672531200,
            nanos: 0,
            price: 50000.0,
            size: 0.5,
            side: TradeSide::Buy,
            maker_order_id: [0; 16],
            taker_order_id: [0; 16],
            exchange_specific: None,
        },
        UnifiedTradeData {
            exchange: ExchangeId::Coinbase,
            symbol: "BTC-USD".to_string(),
            trade_id: 2,
            timestamp: 1672531201,
            nanos: 0,
            price: 50100.0,
            size: 0.3,
            side: TradeSide::Buy,
            maker_order_id: [0; 16],
            taker_order_id: [0; 16],
            exchange_specific: None,
        },
        UnifiedTradeData {
            exchange: ExchangeId::Coinbase,
            symbol: "BTC-USD".to_string(),
            trade_id: 3,
            timestamp: 1672531202,
            nanos: 0,
            price: 49900.0,
            size: 0.2,
            side: TradeSide::Sell,
            maker_order_id: [0; 16],
            taker_order_id: [0; 16],
            exchange_specific: None,
        },
    ];

    for trade in trades {
        engine.process_trade(&trade);
    }

    let analytics = engine.get_analytics("BTC-USD").unwrap();
    assert!((analytics.total_volume - 1.0).abs() < 0.001); // 0.5 + 0.3 + 0.2
    assert_eq!(analytics.trade_count, 3);
    assert_eq!(analytics.buy_count, 2);
    assert_eq!(analytics.sell_count, 1);
    assert_eq!(analytics.high_price, 50100.0);
    assert_eq!(analytics.low_price, 49900.0);
    assert_eq!(analytics.last_price, 49900.0);

    // VWAP calculation
    let expected_vwap = (50000.0 * 0.5 + 50100.0 * 0.3 + 49900.0 * 0.2) / 1.0;
    assert!((analytics.vwap - expected_vwap).abs() < 0.01);
}

#[tokio::test]
async fn test_exchange_message_flow() {
    let (tx, mut rx) = mpsc::channel(100);

    // Simulate exchange sending messages
    tx.send(Message::MarketData(UnifiedMarketData {
        exchange: ExchangeId::Coinbase,
        symbol: "BTC-USD".to_string(),
        timestamp: 1672531200,
        nanos: 0,
        price: 50000.0,
        volume: 0.1,
        side: TradeSide::Buy,
        best_bid: 49999.0,
        best_ask: 50001.0,
        exchange_specific: None,
    }))
    .await
    .unwrap();

    tx.send(Message::Trade(UnifiedTradeData {
        exchange: ExchangeId::Binance,
        symbol: "ETH-USDT".to_string(),
        trade_id: 123456,
        timestamp: 1672531200,
        nanos: 0,
        price: 3000.0,
        size: 0.5,
        side: TradeSide::Sell,
        maker_order_id: [0; 16],
        taker_order_id: [0; 16],
        exchange_specific: None,
    }))
    .await
    .unwrap();

    tx.send(Message::Heartbeat).await.unwrap();
    tx.send(Message::Error("Test error".to_string()))
        .await
        .unwrap();

    // Verify messages are received correctly
    let mut message_count = 0;
    let mut market_data_count = 0;
    let mut trade_count = 0;
    let mut heartbeat_count = 0;
    let mut error_count = 0;

    while let Ok(msg) = rx.try_recv() {
        message_count += 1;
        match msg {
            Message::MarketData(_) => market_data_count += 1,
            Message::Trade(_) => trade_count += 1,
            Message::Heartbeat => heartbeat_count += 1,
            Message::Error(_) => error_count += 1,
        }
    }

    assert_eq!(message_count, 4);
    assert_eq!(market_data_count, 1);
    assert_eq!(trade_count, 1);
    assert_eq!(heartbeat_count, 1);
    assert_eq!(error_count, 1);
}

#[tokio::test]
async fn test_config_loading() {
    // Test default config
    let config = Config::default();
    assert_eq!(config.logger.data_path.to_str().unwrap(), "/mnt/md/data");
    assert_eq!(config.logger.buffer_size, 8192);
    assert_eq!(config.logger.flush_interval_secs, 5);
    assert_eq!(config.logger.health_check_port, 8080);

    assert!(config.exchanges.coinbase.enabled);
    assert!(config.exchanges.binance.enabled);

    assert_eq!(config.exchanges.coinbase.max_connections, 10);
    assert_eq!(config.exchanges.binance.max_connections, 5);

    assert_eq!(config.exchanges.coinbase.symbols_per_connection, 50);
    assert_eq!(config.exchanges.binance.symbols_per_connection, 100);
}

#[tokio::test]
async fn test_symbol_distribution() {
    use logger::exchanges::distribute_symbols;

    let symbols: Vec<String> = (0..255).map(|i| format!("SYMBOL{i}")).collect();
    let distributions = distribute_symbols(symbols.clone(), 50).await;

    // Should have 6 batches (5 full + 1 partial)
    assert_eq!(distributions.len(), 6);

    // First 5 should be full
    for distribution in distributions.iter().take(5) {
        assert_eq!(distribution.len(), 50);
    }

    // Last should have remaining 5
    assert_eq!(distributions[5].len(), 5);

    // Verify all symbols are included
    let mut all_symbols = Vec::new();
    for batch in distributions {
        all_symbols.extend(batch);
    }
    assert_eq!(all_symbols.len(), 255);
}
