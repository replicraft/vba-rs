' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 only the first matching case clause executes
' EXPECT: first
Public Sub Main()
    Select Case 5
        Case 1 To 10
            Debug.Print "first"
        Case 5
            Debug.Print "second"
        Case Else
            Debug.Print "else"
    End Select
End Sub
