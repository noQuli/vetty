export {};

declare global {
  interface Window {
    vettyConfig?: {
      daemon: {
        restUrl: string;
        wsUrl: string;
      };
    };
  }
}
