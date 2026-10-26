' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 numeric output gets a space before and after
' XFAIL: wrong pending
' EXPECT: [ 5 ]
' EXPECT: [ -5 ]
' EXPECT: [ 1.5 ]
' EXPECT: [ 5  6 ]
' EXPECT: [a 7 b]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, 5
    Print #1, -5
    Print #1, 1.5
    Print #1, 5; 6
    Print #1, "a"; 7; "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
