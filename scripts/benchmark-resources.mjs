#!/usr/bin/env node
/**
 * Railbench Resource Benchmark
 * Tests RAM, CPU usage, and throughput for server, core simulation, and large layout processing.
 */

import { performance } from "node:perf_hooks";
import os from "node:os";
import v8 from "node:v8";
import { WebSocket } from "ws";
import http from "node:http";

// Format bytes helper
function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

console.log("==================================================================");
console.log(" 🚆 RAILBENCH HARDWARE & RESOURCE BENCHMARK");
console.log("==================================================================");

// System Hardware Info
const cpus = os.cpus();
const totalMem = os.totalmem();
const freeMem = os.freemem();

console.log(` CPU Model:        ${cpus[0]?.model || "Unknown"} (${cpus.length} Threads)`);
console.log(` System RAM:       ${formatBytes(totalMem - freeMem)} belegt / ${formatBytes(totalMem)} gesamt (${formatBytes(freeMem)} frei)`);
console.log(` Node.js Version:  ${process.version} (V8 ${process.versions.v8})`);
console.log(` Plattform:        ${os.type()} ${os.release()} (${os.arch()})`);
console.log("==================================================================\n");

// 1. Core Data Model & Memory Allocation Benchmark
console.log("--- 1. SPEICHER-BENCHMARK: GLEISPLAN-SKALIERUNG (RAM) ---");

function createBenchmarkPiece(id, x, y, rot) {
  return {
    id: `track-${id}`,
    catalogId: "24188",
    layerId: "default",
    transform: { x, y, rotation: rot },
    connectors: [
      { id: "c1", position: { x, y }, normalAngle: rot, isConnected: false },
      { id: "c2", position: { x: x + 188.3, y }, normalAngle: (rot + 180) % 360, isConnected: false },
    ],
    elevation: { start: 0, end: 0 },
    powerFeed: id % 100 === 0,
    isIsolator: id % 150 === 0,
  };
}

const trackCounts = [500, 1000, 2500, 5000, 10000];

for (const count of trackCounts) {
  if (global.gc) global.gc();
  const memBefore = process.memoryUsage();
  const t0 = performance.now();

  const pieces = [];
  for (let i = 0; i < count; i++) {
    pieces.push(createBenchmarkPiece(i, (i % 50) * 190, Math.floor(i / 50) * 80, 0));
  }

  const jsonString = JSON.stringify({ name: "Benchmark Layout", scale: "H0", pieces });
  const tDuration = performance.now() - t0;
  const memAfter = process.memoryUsage();

  const heapDiff = memAfter.heapUsed - memBefore.heapUsed;
  const jsonSize = Buffer.byteLength(jsonString, "utf8");

  console.log(
    ` • ${count.toString().padStart(5)} Gleise: ` +
      `Generierung: ${tDuration.toFixed(1).padStart(5)} ms | ` +
      `JSON-Größe: ${formatBytes(jsonSize).padStart(8)} | ` +
      `Heap-Bedarf: ~${formatBytes(Math.max(heapDiff, 0)).padStart(8)} ` +
      `(~${(Math.max(heapDiff, 0) / count).toFixed(0)} B/Gleis)`
  );
}

// 2. CPU Benchmark: Snapping- & Abstands-Berechnungen
console.log("\n--- 2. CPU-BENCHMARK: GEOMETRIE & SNAPPING-DURCHSATZ ---");

function benchmarkGeometryCalculations(iterations = 100000) {
  const startCpu = process.cpuUsage();
  const t0 = performance.now();

  let dummySum = 0;
  for (let i = 0; i < iterations; i++) {
    const angle1 = (i * 7.5) % 360;
    const rad = (angle1 * Math.PI) / 180;
    const cosVal = Math.cos(rad);
    const sinVal = Math.sin(rad);

    // Simulated distance calculation
    const dx = i * 0.5 - 100;
    const dy = i * 0.3 - 50;
    const dist = Math.hypot(dx, dy);

    // Angle normalization
    let norm = (angle1 + 180) % 360;
    if (norm < 0) norm += 360;

    dummySum += cosVal + sinVal + dist + norm;
  }

  const duration = performance.now() - t0;
  const cpuDiff = process.cpuUsage(startCpu);
  const totalCpuMs = (cpuDiff.user + cpuDiff.system) / 1000;
  const opsPerSec = (iterations / duration) * 1000;

  console.log(` • Geometrie-Transformationen: ${iterations.toLocaleString()} Durchläufe`);
  console.log(`   Dauer:              ${duration.toFixed(2)} ms`);
  console.log(`   CPU-Zeit (User+Sys): ${totalCpuMs.toFixed(2)} ms`);
  console.log(`   Berechnungs-Tempo:   ${Math.round(opsPerSec).toLocaleString()} Operationen/Sek.`);
}
benchmarkGeometryCalculations();

// 3. Server-Stresstest: Multi-User WebSocket Durchsatz & RAM
console.log("\n--- 3. SERVER-BENCHMARK: MULTI-USER WEBSOCKET STRESS TEST ---");

async function runServerStressTest() {
  // Start server on temporary test port
  const TEST_PORT = 3188;
  process.env.PORT = String(TEST_PORT);
  const serverModule = await import("../server.mjs");

  // Give server time to bind
  await new Promise((r) => setTimeout(r, 600));

  const clientCount = 20;
  const updatesPerClient = 50;
  const clients = [];

  const startMem = process.memoryUsage();
  const startCpu = process.cpuUsage();
  const t0 = performance.now();

  let messagesReceived = 0;
  const totalExpectedBroadcasts = clientCount * (clientCount - 1) * updatesPerClient;

  // Connect multiple clients simultaneously
  const connectPromises = [];
  for (let i = 0; i < clientCount; i++) {
    const p = new Promise((resolve, reject) => {
      const ws = new WebSocket(`ws://localhost:${TEST_PORT}/collab`);
      ws.on("open", () => {
        ws.send(
          JSON.stringify({
            type: "join",
            room: "bench-room",
            user: { id: `bench-user-${i}`, name: `Planer ${i}`, color: "#3b82f6" },
          })
        );
        clients.push(ws);
        resolve();
      });
      ws.on("message", (raw) => {
        const msg = JSON.parse(raw.toString());
        if (msg.type === "cursor") {
          messagesReceived++;
        }
      });
      ws.onerror = reject;
    });
    connectPromises.push(p);
  }

  await Promise.all(connectPromises);
  const tConnect = performance.now() - t0;
  console.log(` • ${clientCount} simultane Clients verbunden in ${tConnect.toFixed(1)} ms`);

  // Flood updates (cursor movements + layout edits)
  const burstStart = performance.now();
  for (let step = 0; step < updatesPerClient; step++) {
    for (let c = 0; c < clientCount; c++) {
      clients[c].send(
        JSON.stringify({
          type: "cursor",
          x: Math.round(100 + step * 2),
          y: Math.round(200 + c * 5),
        })
      );
    }
    await new Promise((r) => setTimeout(r, 5));
  }

  // Wait for queue flush
  await new Promise((r) => setTimeout(r, 400));
  const burstDuration = performance.now() - burstStart;
  const endMem = process.memoryUsage();
  const endCpu = process.cpuUsage(startCpu);
  const totalCpuMs = (endCpu.user + endCpu.system) / 1000;

  // Close test clients
  for (const ws of clients) {
    ws.close();
  }

  console.log(` • Übertragene Nachrichten:   ${(clientCount * updatesPerClient).toLocaleString()} gesendet, ${messagesReceived.toLocaleString()} empfangen`);
  console.log(` • Durchsatz:                 ${Math.round(((clientCount * updatesPerClient) / burstDuration) * 1000).toLocaleString()} Nachrichten/Sek.`);
  console.log(` • Server-CPU-Zeit:           ${totalCpuMs.toFixed(2)} ms`);
  console.log(` • Server-RAM (RSS):          ${formatBytes(endMem.rss)}`);
  console.log(` • Server-RAM (Heap Used):    ${formatBytes(endMem.heapUsed)}`);
  console.log(` • RAM-Zuwachs im Stresstest: ${formatBytes(Math.max(endMem.heapUsed - startMem.heapUsed, 0))}`);

  console.log("\n==================================================================");
  console.log(" ✅ RESSOURCEN-BENCHMARK ERFOLGREICH ABGESCHLOSSEN");
  console.log("==================================================================");

  process.exit(0);
}

runServerStressTest().catch((err) => {
  console.error("Benchmark-Fehler:", err);
  process.exit(1);
});
