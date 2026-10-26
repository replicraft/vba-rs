' INV: func.fileattr
' SOURCE: spec ms-vbal 6.1.2.5.1.4 FileAttr mode values
' EXPECT:  2 
' EXPECT:  1 
' EXPECT:  8 
' EXPECT:  32 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/fa.txt" For Output As #1
    Debug.Print FileAttr(1, 1)
    Close #1
    Open "/fa.txt" For Input As #1
    Debug.Print FileAttr(1, 1)
    Close #1
    Open "/fa.txt" For Append As #1
    Debug.Print FileAttr(1, 1)
    Close #1
    Open "/fa.txt" For Binary As #1
    Debug.Print FileAttr(1, 1)
    Close #1
End Sub
