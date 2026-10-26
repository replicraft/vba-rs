' INV: vbal.5.4.2.9, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 if-with-empty-then: an Else clause directly after Then
' EXPECT: else-ran
Public Sub Main()
    If True Then Else Debug.Print "no"
    If False Then Else Debug.Print "else-ran"
End Sub
