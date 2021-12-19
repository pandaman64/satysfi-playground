import type { GetStaticPaths, GetStaticProps, NextPage } from 'next'
import EditorPage from '../../components/EditorPage'

type Props = {
  input: string,
  stdout: string | null,
  stderr: string | null,
  existsPdf: boolean,
  pdfUrl: string,
}

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

export const getStaticProps: GetStaticProps<Props> = async (context) => {
  const buildId = context.params?.buildId
  console.log(buildId)

  // aid for type inference
  if (typeof buildId !== "string") {
    return {
      notFound: true,
    }
  }

  const s3BaseUrl = process.env.S3_PUBLIC_ENDPOINT
  if (s3BaseUrl === undefined) {
    console.error('S3_PUBLIC_ENDPOINT is not set')
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
  if (input === null) {
    return {
      notFound: true,
    }
  }

  return {
    props: {
      input,
      stdout,
      stderr,
      existsPdf: headPdf !== null,
      pdfUrl: `${s3BaseUrl}/${buildId}/document.pdf`
    }
  }
}

export const getStaticPaths: GetStaticPaths = async (context) => {
  return {
    paths: [],
    fallback: "blocking",
  }
}

const SharePage: NextPage<Props> = (props: Props) => {
  return EditorPage(props);
}

export default SharePage
