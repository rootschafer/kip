import { GoogleGenAI, Type } from "@google/genai";
import { FileNode, SyncRule } from "../types";

// Helper to safely get the API key
const getApiKey = (): string | undefined => {
  return process.env.API_KEY;
};

export const analyzeFileSystem = async (
  nodes: FileNode[],
  syncRules: SyncRule[]
): Promise<string> => {
  const key = getApiKey();
  if (!key) return "API Key missing. Cannot perform AI analysis.";

  const ai = new GoogleGenAI({ apiKey: key });

  const systemPrompt = `
    You are an intelligent file system administrator assistant called Nexus AI.
    Analyze the provided JSON structure representing a user's file ecosystem and sync rules.
    Identify potential issues such as:
    1. Redundant sync loops.
    2. Unprotected critical data (devices with no syncs).
    3. Suggest optimizations.
    Keep the response concise, formatted in Markdown.
  `;

  const dataContext = JSON.stringify({
    deviceCount: nodes.filter(n => n.type === 'device').length,
    totalFiles: nodes.length,
    syncRules: syncRules.map(r => ({ from: r.sourceId, to: r.targetId, schedule: r.schedule })),
    nodesSample: nodes.slice(0, 10).map(n => ({ name: n.name, type: n.type, status: n.status }))
  });

  try {
    const response = await ai.models.generateContent({
      model: 'gemini-3-flash-preview',
      contents: `System Context: ${dataContext}\n\nPlease analyze this configuration.`,
      config: {
        systemInstruction: systemPrompt,
      }
    });

    return response.text || "No analysis available.";
  } catch (error) {
    console.error("Gemini Analysis Error:", error);
    return "Failed to contact Gemini AI service.";
  }
};
