#!/usr/bin/env python3
"""
Generate symbol configuration files for each exchange based on actual data directories.
This script scans /mnt/md/data/{exchange}/ directories and creates JSON configs.
"""

import os
import json
import re
from pathlib import Path

# Base data directory
DATA_DIR = "/mnt/md/data"
CONFIG_DIR = "/home/xander/projects/gpu-charts/server/src/symbols/configs"

def parse_symbol_parts(symbol, exchange):
    """Parse symbol into base and quote currencies based on exchange format."""
    
    if exchange == "binance":
        # Binance uses concatenated format like BTCUSDT, ETHUSDT
        # Common quote currencies: USDT, USDC, BTC, ETH, BNB, BUSD, FDUSD, TRY, EUR, BRL, JPY
        patterns = [
            (r'^(.+)(USDT|USDC|BUSD|FDUSD)$', lambda m: (m.group(1), 'USD')),
            (r'^(.+)(BTC|ETH|BNB)$', lambda m: (m.group(1), m.group(2))),
            (r'^(.+)(EUR|GBP|AUD|CAD|JPY|TRY|BRL)$', lambda m: (m.group(1), m.group(2))),
        ]
        
    elif exchange == "coinbase":
        # Coinbase uses hyphenated format like BTC-USD, ETH-USD
        if '-' in symbol:
            parts = symbol.split('-')
            if len(parts) == 2:
                base, quote = parts
                # Normalize USDT/USDC to USD for display
                if quote in ['USDT', 'USDC']:
                    return base, 'USD'
                return base, quote
                
    elif exchange == "bitfinex":
        # Bitfinex uses format like tBTCUSD, tETHUSD
        if symbol.startswith('t'):
            symbol = symbol[1:]  # Remove 't' prefix
            # Handle special cases with colons
            if ':' in symbol:
                parts = symbol.split(':')
                if len(parts) == 2:
                    base = parts[0]
                    quote = 'USD' if parts[1] in ['USD', 'UST'] else parts[1]
                    return base, quote
            # Standard format
            patterns = [
                (r'^(.+)(USD|UST|EUR|GBP|JPY|BTC|ETH)$', lambda m: (m.group(1), 'USD' if m.group(2) in ['USD', 'UST'] else m.group(2))),
            ]
            
    elif exchange == "kraken":
        # Kraken uses underscore format like XBT_USD, ETH_USD
        if '_' in symbol:
            parts = symbol.split('_')
            if len(parts) == 2:
                base, quote = parts
                # Convert XBT to BTC for normalization
                if base == 'XBT':
                    base = 'BTC'
                # Normalize USDT/USDC to USD
                if quote in ['USDT', 'USDC']:
                    return base, 'USD'
                return base, quote
                
    elif exchange == "okx":
        # OKX uses hyphenated format like BTC-USDT, ETH-USDT
        if '-' in symbol:
            parts = symbol.split('-')
            if len(parts) == 2:
                base, quote = parts
                # Normalize USDT/USDC to USD
                if quote in ['USDT', 'USDC']:
                    return base, 'USD'
                return base, quote
    
    # Try to match patterns for exchanges that use them
    if exchange in ["binance", "bitfinex"]:
        for pattern, extractor in patterns:
            match = re.match(pattern, symbol, re.IGNORECASE)
            if match:
                return extractor(match)
    
    # Fallback: return as-is
    return symbol, "UNKNOWN"

def get_currency_name(symbol):
    """Get full name for a currency symbol."""
    names = {
        'BTC': 'Bitcoin',
        'ETH': 'Ethereum',
        'BNB': 'Binance Coin',
        'SOL': 'Solana',
        'XRP': 'Ripple',
        'ADA': 'Cardano',
        'AVAX': 'Avalanche',
        'DOGE': 'Dogecoin',
        'DOT': 'Polkadot',
        'MATIC': 'Polygon',
        'LINK': 'Chainlink',
        'UNI': 'Uniswap',
        'LTC': 'Litecoin',
        'BCH': 'Bitcoin Cash',
        'ATOM': 'Cosmos',
        'FIL': 'Filecoin',
        'APT': 'Aptos',
        'ARB': 'Arbitrum',
        'OP': 'Optimism',
        'NEAR': 'NEAR Protocol',
        'ALGO': 'Algorand',
        'ICP': 'Internet Computer',
        'FTM': 'Fantom',
        'XLM': 'Stellar',
        'VET': 'VeChain',
        'SAND': 'The Sandbox',
        'MANA': 'Decentraland',
        'AAVE': 'Aave',
        'CRV': 'Curve',
        'MKR': 'Maker',
        'SNX': 'Synthetix',
        'COMP': 'Compound',
        'ENJ': 'Enjin',
        'ZRX': '0x',
        'BAT': 'Basic Attention Token',
        'CHZ': 'Chiliz',
        'GALA': 'Gala',
        'AXS': 'Axie Infinity',
        'FLOW': 'Flow',
        'THETA': 'Theta',
        'EOS': 'EOS',
        'XTZ': 'Tezos',
        'HBAR': 'Hedera',
        'EGLD': 'MultiversX',
        'QNT': 'Quant',
        'TRX': 'TRON',
        'TON': 'Toncoin',
        'SUI': 'Sui',
        'SEI': 'Sei',
        'INJ': 'Injective',
        'TIA': 'Celestia',
        'JUP': 'Jupiter',
        'PYTH': 'Pyth Network',
        'BONK': 'Bonk',
        'WIF': 'dogwifhat',
        'PEPE': 'Pepe',
        'SHIB': 'Shiba Inu',
        'FLOKI': 'Floki',
        # Quote currencies
        'USD': 'US Dollar',
        'EUR': 'Euro',
        'GBP': 'British Pound',
        'JPY': 'Japanese Yen',
        'AUD': 'Australian Dollar',
        'CAD': 'Canadian Dollar',
        'CHF': 'Swiss Franc',
        'TRY': 'Turkish Lira',
        'BRL': 'Brazilian Real',
        'USDT': 'Tether',
        'USDC': 'USD Coin',
        'BUSD': 'Binance USD',
        'DAI': 'Dai',
    }
    return names.get(symbol.upper(), symbol)

def get_tags_for_symbol(base, quote):
    """Generate relevant tags for a trading pair."""
    tags = []
    
    # Add base currency tags
    base_lower = base.lower()
    tags.append(base_lower)
    
    # Add full name as tag
    base_name = get_currency_name(base)
    if base_name != base:
        tags.append(base_name.lower())
    
    # Category tags based on the asset
    categories = {
        'major': ['BTC', 'ETH', 'BNB'],
        'defi': ['UNI', 'AAVE', 'CRV', 'MKR', 'SNX', 'COMP', 'SUSHI', 'YFI'],
        'layer2': ['ARB', 'OP', 'MATIC', 'IMX', 'STRK'],
        'layer1': ['SOL', 'AVAX', 'DOT', 'NEAR', 'ATOM', 'FTM', 'ALGO', 'HBAR'],
        'meme': ['DOGE', 'SHIB', 'PEPE', 'FLOKI', 'BONK', 'WIF'],
        'gaming': ['SAND', 'MANA', 'AXS', 'GALA', 'ENJ', 'IMX'],
        'oracle': ['LINK', 'PYTH', 'API3', 'BAND'],
        'storage': ['FIL', 'AR', 'STORJ'],
        'privacy': ['XMR', 'ZEC', 'DASH'],
        'exchange': ['BNB', 'FTT', 'OKB', 'CRO', 'KCS'],
    }
    
    for category, symbols in categories.items():
        if base.upper() in symbols:
            tags.append(category)
    
    # Add quote currency tags
    quote_lower = quote.lower()
    tags.append(quote_lower)
    
    if quote in ['USD', 'USDT', 'USDC', 'BUSD', 'FDUSD']:
        tags.append('usd')
        if quote != 'USD':
            tags.append('stablecoin')
            tags.append('tether' if quote == 'USDT' else quote_lower)
    elif quote in ['EUR', 'GBP', 'JPY', 'AUD', 'CAD']:
        tags.append('fiat')
        tags.append(get_currency_name(quote).lower())
    
    # Remove duplicates while preserving order
    seen = set()
    unique_tags = []
    for tag in tags:
        if tag not in seen:
            seen.add(tag)
            unique_tags.append(tag)
    
    return unique_tags

def determine_category(base):
    """Determine the category for an asset."""
    # Most assets are crypto
    fiat = ['USD', 'EUR', 'GBP', 'JPY', 'AUD', 'CAD', 'CHF', 'TRY', 'BRL']
    commodities = ['XAU', 'XAG', 'OIL']
    
    if base.upper() in fiat:
        return 'forex'
    elif base.upper() in commodities:
        return 'commodity'
    else:
        return 'crypto'

def generate_config_for_exchange(exchange):
    """Generate configuration for a specific exchange."""
    exchange_dir = Path(DATA_DIR) / exchange
    
    if not exchange_dir.exists():
        print(f"Directory {exchange_dir} does not exist")
        return {}
    
    config = {}
    symbols = sorted([d.name for d in exchange_dir.iterdir() if d.is_dir()])
    
    print(f"\nProcessing {exchange}: {len(symbols)} symbols")
    
    for symbol in symbols:
        base, quote = parse_symbol_parts(symbol, exchange)
        
        if quote == "UNKNOWN":
            print(f"  Warning: Could not parse {symbol}")
            continue
        
        # Create normalized ID
        normalized_id = f"{base}/{quote}"
        
        # Get display names
        base_name = get_currency_name(base)
        quote_name = get_currency_name(quote)
        
        # Generate metadata
        config[symbol] = {
            "normalized_id": normalized_id,
            "base": base,
            "quote": quote,
            "display_name": f"{base_name} / {quote_name}",
            "description": f"{base_name} to {quote_name} {'spot' if determine_category(base) == 'crypto' else ''} trading pair".strip(),
            "tags": get_tags_for_symbol(base, quote),
            "category": determine_category(base)
        }
    
    return config

def main():
    """Main function to generate all exchange configs."""
    exchanges = ['binance', 'coinbase', 'bitfinex', 'kraken', 'okx']
    
    # Create config directory if it doesn't exist
    Path(CONFIG_DIR).mkdir(parents=True, exist_ok=True)
    
    for exchange in exchanges:
        print(f"\n{'='*50}")
        print(f"Generating config for {exchange}")
        print(f"{'='*50}")
        
        config = generate_config_for_exchange(exchange)
        
        if config:
            output_file = Path(CONFIG_DIR) / f"{exchange}.json"
            with open(output_file, 'w') as f:
                json.dump(config, f, indent=2)
            print(f"✓ Saved {len(config)} symbols to {output_file}")
        else:
            print(f"✗ No symbols found for {exchange}")
    
    print(f"\n{'='*50}")
    print("Configuration generation complete!")
    print(f"{'='*50}")

if __name__ == "__main__":
    main()