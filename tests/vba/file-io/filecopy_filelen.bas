' INV: stmt.filecopy, func.filelen, stmt.print
' SOURCE: spec ms-vbal 6.1.2.5.2.3 FileCopy and 6.1.2.5.1.6 FileLen (with 5.4.5.8 Print trailing semicolon)
' EXPECT:  5 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/f1.txt" For Output As #1
    Print #1, "12345";
    Close #1
    FileCopy "/f1.txt", "/f2.txt"
    Debug.Print FileLen("/f2.txt")
End Sub
