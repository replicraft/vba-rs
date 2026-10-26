' INV: stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.8 If statement
' EXPECT: one
' EXPECT: two
' EXPECT: three
' EXPECT: other
Public Sub Main()
    Dim i As Long
    For i = 1 To 4
        If i = 1 Then
            Debug.Print "one"
        ElseIf i = 2 Then
            Debug.Print "two"
        ElseIf i = 3 Then
            Debug.Print "three"
        Else
            Debug.Print "other"
        End If
    Next
End Sub
