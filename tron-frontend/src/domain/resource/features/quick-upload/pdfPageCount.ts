interface PdfDocument {
  destroy?: () => Promise<void>;
  numPages: number;
}

interface PdfLoadingTask {
  destroy: () => Promise<void>;
  promise: Promise<PdfDocument>;
}

export interface PdfJsModule {
  GlobalWorkerOptions: { workerSrc: string };
  getDocument: (source: { data: Uint8Array }) => PdfLoadingTask;
}

export type PdfJsLoader = () => Promise<PdfJsModule>;

let pdfJsPromise: Promise<PdfJsModule> | null = null;

async function loadPdfJs(): Promise<PdfJsModule> {
  if (!pdfJsPromise) {
    pdfJsPromise = Promise.all([
      import("pdfjs-dist"),
      import("pdfjs-dist/build/pdf.worker.mjs?url"),
    ]).then(([pdfjs, worker]) => {
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      return pdfjs as PdfJsModule;
    }).catch((cause: unknown) => {
      pdfJsPromise = null;
      throw cause;
    });
  }
  return pdfJsPromise;
}

/**
 * Reads only the PDF document metadata required by the built-in PDF Resource
 * definition. It is deliberately lazy so ordinary file/image uploads do not
 * load PDF.js or its worker.
 */
export async function readPdfPageCount(file: File, loader: PdfJsLoader = loadPdfJs) {
  const pdfjs = await loader();
  const loadingTask = pdfjs.getDocument({ data: new Uint8Array(await file.arrayBuffer()) });
  let document: PdfDocument | null = null;
  try {
    document = await loadingTask.promise;
    return document.numPages;
  } finally {
    await document?.destroy?.();
    await loadingTask.destroy();
  }
}
