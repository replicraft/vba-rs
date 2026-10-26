' INV: stmt.if-then-else
' SOURCE: derived nesting of blocks
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
