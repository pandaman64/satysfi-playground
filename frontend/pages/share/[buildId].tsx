import type { GetStaticPaths, GetStaticProps, NextPage } from 'next'
import EditorPage from '../../components/EditorPage'
import getEditorPageProps, { EditorPageProps } from '../../lib/getEditorPageProps'

export const getStaticProps: GetStaticProps<EditorPageProps> = async (context) => {
  const buildId = context.params?.buildId

  // aid for type inference
  if (typeof buildId !== "string") {
    return {
      notFound: true,
    }
  }

  return await getEditorPageProps(buildId)
}

export const getStaticPaths: GetStaticPaths = async (context) => {
  return {
    paths: [],
    fallback: "blocking",
  }
}

const SharePage: NextPage<EditorPageProps> = (props: EditorPageProps) => {
  return EditorPage(props)
}

export default SharePage
