import type { GetStaticPaths, GetStaticProps, NextPage } from 'next'
import EditorPage from '../components/EditorPage'

type Props = {
  apiUrl: string,
}

const Home: NextPage<Props> = ({ apiUrl }: Props) => {
  return EditorPage({
    input: "",
    stdout: null,
    stderr: null,
    existsPdf: false,
    pdfUrl: null,
    apiUrl,
  })
}

export const getStaticProps: GetStaticProps<Props> = async (context) => {
  const apiUrl = process.env.API_ENDPOINT
  if (apiUrl === undefined) {
    console.error(`Environment variable is not set: API_ENDPOINT=${apiUrl}`)
    return {
      notFound: true,
    }
  }

  return {
    props: {
      apiUrl,
    }
  }
}

export default Home
