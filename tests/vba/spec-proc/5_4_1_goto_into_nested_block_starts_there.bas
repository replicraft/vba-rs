' INV: vbal.5.4.1, stmt.goto
' SOURCE: spec ms-vbal 5.4.1 a jump into a block executes from the target label; earlier statements are skipped
' EXPECT: in
' EXPECT: out
Public Sub Main()
    GoTo Inside
    If False Then
        Debug.Print "skipped"
Inside:
        Debug.Print "in"
    End If
    Debug.Print "out"
End Sub
