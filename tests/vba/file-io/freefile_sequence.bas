' INV: func.freefile, stmt.open, stmt.close
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/freefile-function
' EXPECT:  1  2 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Integer, b As Integer
    a = FreeFile
    Open "/t/1" For Output As #a
    b = FreeFile
    Debug.Print a; b
    Close #a
End Sub
