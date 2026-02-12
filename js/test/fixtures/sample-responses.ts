/**
 * Canned browsy API responses for testing.
 */

export const SAMPLE_BROWSE_RESPONSE =
  'title: Example Page\nurl: https://example.com\nels: 5\n---\n' +
  '[1:a "Home" @top-L]\n' +
  '[2:a "About" @top-R]\n' +
  '[3:h1 "Welcome to Example"]\n' +
  '[4:p "This is an example page."]\n' +
  '[5:button "Click Me"]\n';

export const SAMPLE_SEARCH_RESPONSE = [
  {
    title: "Example Domain",
    url: "https://example.com",
    snippet: "This domain is for use in illustrative examples.",
  },
  {
    title: "Wikipedia - Example",
    url: "https://en.wikipedia.org/wiki/Example",
    snippet: "An example is a thing serving to illustrate or explain.",
  },
];

export const SAMPLE_PAGE_INFO = {
  title: "Example Page",
  url: "https://example.com",
  page_type: "Content",
  suggested_actions: [],
  alerts: [],
  pagination: null,
};

export const SAMPLE_TABLES = [
  {
    headers: ["Name", "Value"],
    rows: [
      ["Alpha", "100"],
      ["Beta", "200"],
    ],
  },
];
