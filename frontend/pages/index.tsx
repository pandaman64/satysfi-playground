import type { NextPage } from 'next'
import EditorPage from '../components/EditorPage'

const Home: NextPage = () => {
  return EditorPage({
    input: "",
    stdout: null,
    stderr: null,
    existsPdf: false,
    pdfUrl: null,
  })
}

export default Home
