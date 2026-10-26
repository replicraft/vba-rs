' INV: stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 Select Case statement first matching clause
' EXPECT: first
Public Sub Main()
    Select Case 3
        Case 1 To 5
            Debug.Print "first"
        Case 3
            Debug.Print "second"
    End Select
End Sub
