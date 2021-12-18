import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import EditorPage from '../../components/EditorPage'

const SharePage: NextPage = () => {
  const router = useRouter()
  const { buildId } = router.query

  return EditorPage({
    s3Url: `http://localhost:9000/satysfi-playground/${buildId}`
  });
}

export default SharePage