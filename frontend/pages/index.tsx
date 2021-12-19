import type { NextPage } from 'next'
import EditorPage from '../components/EditorPage'

const Home: NextPage = () => {
  return EditorPage({
    input: "",
    stdout: null,
    stderr: null,
    pdfUrl: null,
  })
}

export default Home
