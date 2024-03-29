import monaco from 'monaco-editor'
import Editor from "@monaco-editor/react"
import { useRef, useState, VFC } from "react"
import { Button, Tab, TabList, TabPanel, TabPanels, Tabs, Textarea } from '@chakra-ui/react'
import Head from 'next/head'
import { useRouter } from 'next/router'
import styles from '../styles/Home.module.css'
import { EditorPageProps } from '../lib/getEditorPageProps'

function generatePdfIframe(pdfUrl: string): JSX.Element {
  return <iframe src={pdfUrl} width="100%" height="100%"></iframe>
}

const EditorPage: VFC<EditorPageProps> = ({ input, stdout, stderr, existsPdf, pdfUrl, apiUrl }: EditorPageProps) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const router = useRouter()

  async function onRun() {
    alert("SATySFi Playgroundはコンパイル機能を停止しました。これまでのご利用ありがとうございました。")
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
              <Textarea resize="none" width="100%" height="100%" value={stdout ?? ""}></Textarea>
            </TabPanel>
            <TabPanel padding={0} height="100%">
              <Textarea resize="none" width="100%" height="100%" value={stderr ?? ""}></Textarea>
            </TabPanel>
          </TabPanels>
        </Tabs>
      </div>
    </div>
  )
}

export default EditorPage