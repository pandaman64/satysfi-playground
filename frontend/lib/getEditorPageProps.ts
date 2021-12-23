import { GetStaticPropsResult } from "next"

function textIfOk(response: Response): Promise<string | null> {
  if (response.ok) {
    return response.text()
  } else {
    return Promise.resolve(null)
  }
}

function extractResult<T>(result: PromiseSettledResult<T>): T | null {
  if (result.status === "fulfilled") {
    return result.value
  } else {
    return null
  }
}

export type EditorPageProps = {
  input: string | null,
  stdout: string | null,
  stderr: string | null,
  existsPdf: boolean,
  pdfUrl: string | null,
  apiUrl: string,
}

async function getEditorPageProps(buildId: string): Promise<GetStaticPropsResult<EditorPageProps>> {
  console.log(`frontend: buildId = ${buildId}`)
  const s3BaseUrl = process.env.S3_PUBLIC_ENDPOINT
  const apiUrl = process.env.API_ENDPOINT
  if (s3BaseUrl === undefined || apiUrl === undefined) {
    console.error(`Environment variables are not set: S3_PUBLIC_ENDPOINT=${s3BaseUrl}, API_ENDPOINT=${apiUrl}`)
    return {
      notFound: true,
    }
  }

  const [headPdf, input, stdout, stderr] = await Promise.allSettled([
    fetch(`${s3BaseUrl}/${buildId}/document.pdf`, {
      method: "HEAD",
    }).then(textIfOk),
    fetch(`${s3BaseUrl}/${buildId}/input.saty`).then(textIfOk),
    fetch(`${s3BaseUrl}/${buildId}/stdout.txt`).then(textIfOk),
    fetch(`${s3BaseUrl}/${buildId}/stderr.txt`).then(textIfOk),
  ]).then(args => args.map(extractResult))

  return {
    props: {
      input,
      stdout,
      stderr,
      existsPdf: headPdf !== null,
      pdfUrl: `${s3BaseUrl}/${buildId}/document.pdf`,
      apiUrl,
    }
  }
}

export default getEditorPageProps