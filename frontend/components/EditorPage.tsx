import monaco from 'monaco-editor'
import Editor from "@monaco-editor/react"
import { useRef, useState, VFC } from "react"
import { Button, Tab, TabList, TabPanel, TabPanels, Tabs, Textarea } from '@chakra-ui/react'
import Head from 'next/head'
import { useRouter } from 'next/router'
import styles from '../styles/Home.module.css'

function generatePdfIframe(pdfUrl: string): JSX.Element {
  return <iframe src={pdfUrl} width="100%" height="100%"></iframe>
}

type EditorPageProps = {
  input: string | null,
  stdout: string | null,
  stderr: string | null,
  existsPdf: boolean,
  pdfUrl: string | null,
  apiUrl: string,
}

const EditorPage: VFC<EditorPageProps> = ({ input, stdout, stderr, existsPdf, pdfUrl, apiUrl }: EditorPageProps) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const router = useRouter()

  async function onRun() {
    if (editorRef.current !== null) {
      setIsLoading(true);
      try {
        const source = editorRef.current.getValue();
        const body = {
          source: source,
        };
        const response = await fetch(`${apiUrl}/persist`, {
          method: "POST",
          mode: "cors",
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(body),
        });
        const { status, s3_url: s3Url }: { status: number, s3_url: string } = await response.json();

        const buildId = s3Url.split("/").pop()
        if (buildId !== undefined) {
          router.push(`/share/${buildId}`)
        }
      } finally {
        setIsLoading(false);
      }
    }
  }

  return (
    <div className={styles.container}>
      <Head>
        <title>SATySFi Playground</title>
        <meta name="description" content="Playground for SATySFi programming/typesetting language" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <div className={styles.header}>
        <Button isLoading={isLoading} size="lg" colorScheme="blue" onClick={onRun}>
          Run
        </Button>
      </div>

      <div className={styles.editor}>
        <Editor
          width="50%"
          height="100%"
          value={input ?? ""}
          theme="vs-dark"
          options={{
            fontSize: 16,
          }}
          onMount={(editor) => { editorRef.current = editor }}
        />
        <Tabs variant="line" isFitted width="50%" height="100%" display="flex" flexDirection="column" defaultIndex={existsPdf ? 0 : 1}>
          <TabList>
            <Tab>PDF</Tab>
            <Tab>stdout</Tab>
            <Tab>stderr</Tab>
          </TabList>
          <TabPanels flex={1}>
            <TabPanel padding={0} height="100%">{pdfUrl ? generatePdfIframe(pdfUrl) : <></>}</TabPanel>
            <TabPanel padding={0} height="100%">
              <Textarea isReadOnly resize="none" width="100%" height="100%" value={stdout ?? ""}></Textarea>
            </TabPanel>
            <TabPanel padding={0} height="100%">
              <Textarea isReadOnly resize="none" width="100%" height="100%" value={stderr ?? ""}></Textarea>
            </TabPanel>
          </TabPanels>
        </Tabs>
      </div>
    </div>
  )
}

export default EditorPage