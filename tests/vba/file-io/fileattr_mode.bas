' INV: func.fileattr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fileattr-function
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
