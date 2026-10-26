' INV: vbal.5.4.1.1, stmt.goto
' SOURCE: spec ms-vbal 5.4.1.1 line-number labels are labels; execution continues after the label
' EXPECT: ten
' EXPECT: thirty
Public Sub Main()
    GoTo 10
5   Debug.Print "skipped"
10  Debug.Print "ten"
    GoTo 30
20: Debug.Print "skipped too"
30: Debug.Print "thirty"
End Sub
