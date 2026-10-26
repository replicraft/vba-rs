' INV: func.input, stmt.print
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/input-function the $ and B forms read characters (bytes) from a file opened for Input or Binary
' GRANT: file-read
' GRANT: file-write
' EXPECT: [hello]
' EXPECT: [ ]
' EXPECT: [wo]
' EXPECT: [rl]
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/i.txt" For Output As #f
    Print #f, "hello world"
    Close #f
    Open "/t/i.txt" For Input As #f
    s = Input$(5, #f)
    Debug.Print "[" & s & "]"
    s = Input(1, #f)
    Debug.Print "[" & s & "]"
    s = InputB$(2, #f)
    Debug.Print "[" & s & "]"
    s = InputB(2, f)
    Debug.Print "[" & s & "]"
    Close #f
End Sub
