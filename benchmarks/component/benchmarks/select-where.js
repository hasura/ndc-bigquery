import {check} from "k6";
import http from "k6/http";

const testid = "select-where";
const url = `http://agent:8100/query`;
const data = {
  table : "Album",
  query : {
    fields : {
      id : {type : "column", column : "AlbumId", arguments : {}},
      title : {type : "column", column : "Title", arguments : {}},
      artist_id : {type : "column", column : "ArtistId", arguments : {}},
    },
    where : {
      type : "binary_comparison_operator",
      column : {
        type : "column",
        name : "Title",
        path : [],
      },
      operator : {
        type : "other",
        name : "ilike",
      },
      value : {
        type : "scalar",
        value : "%a%",
      },
    },
  },
  arguments : {},
  table_relationships : {},
};

export default function() {
  const response = http.post(url, JSON.stringify(data), {
    headers : {
      "Content-Type" : "application/json",
    },
  });

  check(response, {
    "status is 200" : (r) => r.status == 200,
  });
}

export function handleSummary(data) {
  const outputDirectory = __ENV.OUTPUT_DIRECTORY;
  if (outputDirectory) {
    const summaryFile = `${outputDirectory}/summaries/${testid}__${
        new Date().toISOString()}.json`;
    return {
      [summaryFile] : JSON.stringify(data),
    };
  }
}

export const options = {
  tags : {
    testid,
  },
  scenarios : {
    short_sustained : {
      executor : "constant-vus",
      vus : 100,
      duration : "10s",
    },
  },
};
