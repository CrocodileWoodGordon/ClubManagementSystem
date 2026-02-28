const apiProxyTarget =
  (process.env.API_PROXY_TARGET ?? "http://localhost:8080").replace(/\/$/, "");

const nextConfig = {
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${apiProxyTarget}/api/:path*`,
      },
    ];
  },
};

export default nextConfig;
