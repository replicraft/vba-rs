' INV: stmt.write, stmt.input, stmt.open, stmt.close
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/inputstatement
' EXPECT: text 42  2.5 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, a As String, b As Long, c As Double
    f = FreeFile
    Open "/t/w.txt" For Output As #f
    Write #f, "text", 42, 2.5
    Close #f
    Open "/t/w.txt" For Input As #f
    Input #f, a, b, c
    Close #f
    Debug.Print a; b; c
End Sub
