' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
' EXPECT: done
Public Sub Main()
    Select Case 9
        Case 1
            Debug.Print "x"
    End Select
    Debug.Print "done"
End Sub
