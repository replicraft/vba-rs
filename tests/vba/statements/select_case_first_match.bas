' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
' EXPECT: first
Public Sub Main()
    Select Case 3
        Case 1 To 5
            Debug.Print "first"
        Case 3
            Debug.Print "second"
    End Select
End Sub
