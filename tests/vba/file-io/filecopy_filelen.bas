' INV: stmt.filecopy, func.filelen, stmt.print
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/filecopy-statement
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
