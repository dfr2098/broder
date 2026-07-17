FROM node:20.14.0-alpine3.20

ENV NODE_ENV=development

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY . .
RUN chown -R node:node /app

USER node

EXPOSE 3000
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0", "--port", "3000"]
