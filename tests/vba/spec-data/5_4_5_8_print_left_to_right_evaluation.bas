' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 output items are processed left to right
' EXPECT: 1 2 3
' EXPECT: [123]
' GRANT: file-read
' GRANT: file-write
Dim log As String
Private Function Mark(ByVal n As String) As String
    log = log & n & " "
    Mark = n
End Function
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, Mark("1"); Mark("2"); Mark("3")
    Close #1
    Debug.Print Trim(log)
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
