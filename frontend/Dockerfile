# syntax=docker/dockerfile:1.6

ARG NEXT_PUBLIC_API_URL=/api
ARG API_PROXY_TARGET=http://localhost:8080

FROM node:20-alpine AS deps

WORKDIR /app
RUN apk add --no-cache libc6-compat

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

FROM node:20-alpine AS builder

WORKDIR /app
RUN apk add --no-cache libc6-compat
ENV NEXT_TELEMETRY_DISABLED=1
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ARG API_PROXY_TARGET
ENV API_PROXY_TARGET=${API_PROXY_TARGET}

COPY --from=deps /app/node_modules ./node_modules
COPY frontend/. .

RUN npm run build

FROM node:20-alpine AS runner

WORKDIR /app
RUN apk add --no-cache libc6-compat
ENV NODE_ENV=production
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ARG API_PROXY_TARGET
ENV API_PROXY_TARGET=${API_PROXY_TARGET}

COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/package-lock.json ./package-lock.json
COPY --from=builder /app/next.config.ts ./next.config.ts

RUN npm ci --omit=dev

EXPOSE 3000

CMD ["npm", "run", "start"]
