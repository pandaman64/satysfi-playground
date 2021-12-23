import type { GetStaticPaths, GetStaticProps, NextPage } from 'next'
import EditorPage from '../components/EditorPage'
import getEditorPageProps, { EditorPageProps } from '../lib/getEditorPageProps'

const Home: NextPage<EditorPageProps> = (props) => {
  return EditorPage(props)
}

export const getStaticProps: GetStaticProps<EditorPageProps> = async (context) => {
  const indexPageBuildId = process.env.INDEX_PAGE_BUILD_ID
  console.log(`INDEX_PAGE_BUILD_ID=${indexPageBuildId}`)
  if (indexPageBuildId === undefined) {
    const apiUrl = process.env.API_ENDPOINT
    if (apiUrl === undefined) {
      console.error(`Environment variable is not set: API_ENDPOINT=${apiUrl}`)
      return {
        notFound: true,
      }
    }

    return {
      props: {
        input: "",
        stdout: null,
        stderr: null,
        existsPdf: false,
        pdfUrl: null,
        apiUrl,
      }
    }
  }

  return await getEditorPageProps(indexPageBuildId)
}

export default Home
