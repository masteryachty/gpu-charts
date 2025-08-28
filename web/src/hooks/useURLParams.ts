import { useMemo, useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

/**
 * Hook for managing URL parameters
 * Provides easy access to URL search params and navigation
 */
export function useURLParams() {
  const location = useLocation();
  const navigate = useNavigate();

  // Parse current URL parameters
  const params = useMemo(() => {
    return new URLSearchParams(location.search);
  }, [location.search]);

  // Get parameter value by key
  const getParam = useCallback((key: string): string | null => {
    return params.get(key);
  }, [params]);

  // Get parameter with default value
  const getParamWithDefault = useCallback((key: string, defaultValue: string): string => {
    return params.get(key) || defaultValue;
  }, [params]);

  // Set parameter and update URL
  const setParam = useCallback((key: string, value: string | null) => {
    const newParams = new URLSearchParams(params);
    
    if (value === null || value === undefined) {
      newParams.delete(key);
    } else {
      newParams.set(key, value);
    }
    
    const newUrl = `${location.pathname}?${newParams.toString()}`;
    navigate(newUrl, { replace: true });
  }, [params, location.pathname, navigate]);

  // Set multiple parameters at once
  const setParams = useCallback((updates: Record<string, string | null>) => {
    const newParams = new URLSearchParams(params);
    
    Object.entries(updates).forEach(([key, value]) => {
      if (value === null || value === undefined) {
        newParams.delete(key);
      } else {
        newParams.set(key, value);
      }
    });
    
    const newUrl = `${location.pathname}?${newParams.toString()}`;
    navigate(newUrl, { replace: true });
  }, [params, location.pathname, navigate]);

  // Parse common chart parameters
  const chartParams = useMemo(() => ({
    topic: getParam('topic') || 'BTC-USD',
    start: parseInt(getParam('start') || '0') || Math.floor(Date.now() / 1000) - 86400,
    end: parseInt(getParam('end') || '0') || Math.floor(Date.now() / 1000),
    preset: getParam('preset') || undefined,
    exchange: getParam('exchange') || 'coinbase',
  }), [getParam]);

  // Update chart parameters
  const setChartParams = useCallback((updates: {
    topic?: string;
    start?: number;
    end?: number;
    preset?: string;
    exchange?: string;
  }) => {
    const paramUpdates: Record<string, string | null> = {};
    
    if (updates.topic !== undefined) paramUpdates.topic = updates.topic;
    if (updates.start !== undefined) paramUpdates.start = updates.start.toString();
    if (updates.end !== undefined) paramUpdates.end = updates.end.toString();
    if (updates.preset !== undefined) paramUpdates.preset = updates.preset;
    if (updates.exchange !== undefined) paramUpdates.exchange = updates.exchange;
    
    setParams(paramUpdates);
  }, [setParams]);

  return {
    // Raw access
    params,
    getParam,
    getParamWithDefault,
    setParam,
    setParams,
    
    // Chart-specific helpers
    chartParams,
    setChartParams,
  };
}