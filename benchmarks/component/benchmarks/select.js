import { check } from "k6";
import http from "k6/http";

const testid = "select";
// const url = `http://localhost:8100/query`;
const url = `http://agent:3000/deployment/${__ENV.DEPLOYMENT_ID}/query`;
const data = {
  table: "Album",
  query: {
    fields: {
      id: { type: "column", column: "AlbumId", arguments: {} },
      title: { type: "column", column: "Title", arguments: {} },
      artist_id: { type: "column", column: "ArtistId", arguments: {} },
    },
  },
  arguments: {},
  table_relationships: {},
};

export default function () {
  const response = http.post(url, JSON.stringify(data), {
    headers: {
      "Content-Type": "application/json",
    },
  });

  check(response, {
    "status is 200": (r) => r.status == 200,
  });
}

export function handleSummary(data) {
  const outputDirectory = __ENV.OUTPUT_DIRECTORY;
  if (outputDirectory) {
    const summaryFile = `${outputDirectory}/summaries/${testid}__${new Date().toISOString()}.json`;
    return {
      [summaryFile]: JSON.stringify(data),
    };
  }
}

export const options = {
  tags: {
    testid,
  },
  scenarios: {
    short_sustained: {
      executor: "constant-vus",
      vus: 100,
      duration: "10s",
    },
  },
};
