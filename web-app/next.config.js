/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // The production build can call the native @opendocu/node binding from a
  // Next.js API route (server components). For the static demo we use the
  // dependency-free JS port in lib/reduce.js so no backend is required.
  experimental: {
    serverActions: {
      bodySizeLimit: '50mb',
    },
  },
};

module.exports = nextConfig;
