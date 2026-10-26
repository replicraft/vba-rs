' INV: stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.8 If statement
' EXPECT: b
Public Sub Main()
    If True Then
        If False Then
            Debug.Print "a"
        Else
            Debug.Print "b"
        End If
    End If
End Sub
