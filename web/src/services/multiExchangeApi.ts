// Multi-Exchange Data API Service

import { parseSymbol } from './symbolApi';

// Get API base URL from environment or use default
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 
  (window.location.hostname === 'localhost' ? 'https://localhost:8443' : 'https://api.rednax.io');

export interface ExchangeDataResponse {
  exchange: string;
  symbol: string;
  columns: Array<{
    name: string;
    record_size: number;
    num_records: number;
    data_length: number;
  }>;
  data?: ArrayBuffer;
  error?: string;
}

export interface MultiExchangeData {
  baseSymbol: string;
  startTime: number;
  endTime: number;
  exchanges: ExchangeDataResponse[];
}

/**
 * Fetch data for a single exchange
 */
async function fetchExchangeData(
  exchange: string,
  baseSymbol: string,
  startTime: number,
  endTime: number,
  columns: string[] = ['time', 'best_bid', 'best_ask', 'price', 'volume']
): Promise<ExchangeDataResponse> {
  const symbol = `${exchange}:${baseSymbol}`;
  
  try {
    const params = new URLSearchParams({
      symbol: baseSymbol,  // Just the base symbol without exchange prefix
      exchange: exchange,  // Exchange as separate parameter
      type: 'MD',
      start: startTime.toString(),
      end: endTime.toString(),
      columns: columns.join(','),
    });
    
    const response = await fetch(`${API_BASE_URL}/api/data?${params}`, {
      method: 'GET',
      headers: {
        'Accept': 'application/octet-stream, application/json',
      },
    });
    
    if (!response.ok) {
      throw new Error(`Failed to fetch data for ${exchange}: ${response.statusText}`);
    }
    
    // Parse the response
    const contentType = response.headers.get('content-type');
    if (contentType?.includes('application/json')) {
      // JSON response with metadata
      const metadata = await response.json();
      return {
        exchange,
        symbol,
        columns: metadata.columns || [],
        error: metadata.error,
      };
    } else {
      // Binary response - read header first
      const blob = await response.blob();
      const arrayBuffer = await blob.arrayBuffer();
      
      // Try to parse JSON header from the beginning of the response
      // The server typically sends JSON metadata followed by binary data
      const decoder = new TextDecoder();
      const view = new Uint8Array(arrayBuffer);
      
      // Find the end of JSON (look for closing brace)
      let jsonEnd = -1;
      for (let i = 0; i < Math.min(view.length, 10000); i++) {
        if (view[i] === 125) { // '}' character
          jsonEnd = i + 1;
          break;
        }
      }
      
      if (jsonEnd > 0) {
        const jsonStr = decoder.decode(view.slice(0, jsonEnd));
        const metadata = JSON.parse(jsonStr);
        const binaryData = arrayBuffer.slice(jsonEnd);
        
        return {
          exchange,
          symbol,
          columns: metadata.columns || [],
          data: binaryData,
        };
      }
      
      // Fallback if no JSON header found
      return {
        exchange,
        symbol,
        columns: columns.map(name => ({
          name,
          record_size: 4,
          num_records: arrayBuffer.byteLength / (4 * columns.length),
          data_length: arrayBuffer.byteLength / columns.length,
        })),
        data: arrayBuffer,
      };
    }
  } catch (error) {
    console.error(`Error fetching data for ${exchange}:`, error);
    return {
      exchange,
      symbol,
      columns: [],
      error: error instanceof Error ? error.message : 'Unknown error',
    };
  }
}

/**
 * Fetch data from multiple exchanges in parallel
 */
export async function fetchMultiExchangeData(
  exchanges: string[],
  baseSymbol: string,
  startTime: number,
  endTime: number,
  columns?: string[]
): Promise<MultiExchangeData> {
  console.log(`Fetching data for ${exchanges.join(', ')} - ${baseSymbol}`);
  
  // Fetch data from all exchanges in parallel
  const promises = exchanges.map(exchange =>
    fetchExchangeData(exchange, baseSymbol, startTime, endTime, columns)
  );
  
  const results = await Promise.allSettled(promises);
  
  // Process results, including both successful and failed fetches
  const exchangeData: ExchangeDataResponse[] = results.map((result, index) => {
    if (result.status === 'fulfilled') {
      return result.value;
    } else {
      // Failed fetch
      return {
        exchange: exchanges[index],
        symbol: `${exchanges[index]}:${baseSymbol}`,
        columns: [],
        error: result.reason?.message || 'Failed to fetch data',
      };
    }
  });
  
  return {
    baseSymbol,
    startTime,
    endTime,
    exchanges: exchangeData,
  };
}

/**
 * Parse binary data from the server response
 */
export function parseBinaryData(
  data: ArrayBuffer,
  columns: string[]
): Map<string, Float32Array> {
  const result = new Map<string, Float32Array>();
  const view = new DataView(data);
  
  const recordSize = columns.length * 4; // 4 bytes per value
  const numRecords = data.byteLength / recordSize;
  
  // Initialize arrays for each column
  columns.forEach(column => {
    result.set(column, new Float32Array(numRecords));
  });
  
  // Parse the data
  for (let record = 0; record < numRecords; record++) {
    for (let col = 0; col < columns.length; col++) {
      const offset = record * recordSize + col * 4;
      const value = view.getFloat32(offset, true); // little-endian
      result.get(columns[col])![record] = value;
    }
  }
  
  return result;
}

/**
 * Align time series data from multiple exchanges
 * Interpolates missing values to create aligned datasets
 */
export function alignTimeSeriesData(
  exchangeDataMap: Map<string, Map<string, Float32Array>>
): Map<string, Map<string, Float32Array>> {
  // Find the common time range across all exchanges
  let minTime = Infinity;
  let maxTime = -Infinity;
  
  exchangeDataMap.forEach(data => {
    const timeData = data.get('time');
    if (timeData && timeData.length > 0) {
      minTime = Math.min(minTime, timeData[0]);
      maxTime = Math.max(maxTime, timeData[timeData.length - 1]);
    }
  });
  
  if (minTime === Infinity || maxTime === -Infinity) {
    return exchangeDataMap; // No data to align
  }
  
  // Create a common time grid (1-second intervals for now)
  const timeStep = 1; // 1 second
  const numPoints = Math.floor((maxTime - minTime) / timeStep) + 1;
  const commonTime = new Float32Array(numPoints);
  
  for (let i = 0; i < numPoints; i++) {
    commonTime[i] = minTime + i * timeStep;
  }
  
  // Interpolate each exchange's data to the common time grid
  const alignedData = new Map<string, Map<string, Float32Array>>();
  
  exchangeDataMap.forEach((data, exchange) => {
    const alignedExchangeData = new Map<string, Float32Array>();
    const originalTime = data.get('time');
    
    if (!originalTime || originalTime.length === 0) {
      alignedData.set(exchange, alignedExchangeData);
      return;
    }
    
    // For each column, interpolate to common time grid
    data.forEach((values, column) => {
      if (column === 'time') {
        alignedExchangeData.set('time', commonTime);
      } else {
        const interpolated = new Float32Array(numPoints);
        
        // Simple linear interpolation
        let originalIndex = 0;
        for (let i = 0; i < numPoints; i++) {
          const targetTime = commonTime[i];
          
          // Find the surrounding points in the original data
          while (originalIndex < originalTime.length - 1 && 
                 originalTime[originalIndex + 1] < targetTime) {
            originalIndex++;
          }
          
          if (originalIndex >= originalTime.length - 1) {
            // Use last value
            interpolated[i] = values[originalTime.length - 1];
          } else if (originalTime[originalIndex] > targetTime) {
            // Use first value
            interpolated[i] = values[0];
          } else {
            // Linear interpolation
            const t1 = originalTime[originalIndex];
            const t2 = originalTime[originalIndex + 1];
            const v1 = values[originalIndex];
            const v2 = values[originalIndex + 1];
            
            const ratio = (targetTime - t1) / (t2 - t1);
            interpolated[i] = v1 + (v2 - v1) * ratio;
          }
        }
        
        alignedExchangeData.set(column, interpolated);
      }
    });
    
    alignedData.set(exchange, alignedExchangeData);
  });
  
  return alignedData;
}