import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
const ORIGINAL_DEMO_MODE = process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;

function makeRequest(url: string, { withSession }: { withSession?: boolean } = {}) {
  const req = new NextRequest(new URL(url, "http://localhost:3000"));
  if (withSession) {
    req.cookies.set("__Host-session", "test-session-cookie");
  }
  return req;
}

function restoreEnv() {
  Object.defineProperty(process.env, "NODE_ENV", {
    configurable: true,
    enumerable: true,
    writable: true,
    value: ORIGINAL_NODE_ENV,
  });

  if (ORIGINAL_BYPASS === undefined) delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  else process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;

  if (ORIGINAL_DEMO_MODE === undefined) delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  else process.env.NEXT_PUBLIC_ARGO_DEMO_MODE = ORIGINAL_DEMO_MODE;
}

describe("root middleware redirect", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  it("redirects unauthenticated bare root traffic to /landing", () => {
    const res = middleware(makeRequest("http://localhost:3000/"));

    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe("http://localhost:3000/landing");
  });

  it("lets authenticated root traffic render the app home", () => {
    const res = middleware(makeRequest("http://localhost:3000/", { withSession: true }));

    expect(res.headers.get("location")).toBeNull();
  });

  it("preserves the local auth-bypass root workflow", () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";

    const res = middleware(makeRequest("http://localhost:3000/"));

    expect(res.headers.get("location")).toBeNull();
  });

  it("redirects demo-mode root traffic until a demo session is minted", () => {
    process.env.NEXT_PUBLIC_ARGO_DEMO_MODE = "1";

    const res = middleware(makeRequest("http://localhost:3000/"));

    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe("http://localhost:3000/landing");
  });
});
