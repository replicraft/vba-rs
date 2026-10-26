' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
' EXPECT: BC
' EXPECT: range
Public Sub Main()
    Select Case "b"
        Case "a"
            Debug.Print "A"
        Case "b", "c"
            Debug.Print "BC"
    End Select
    Select Case "apple"
        Case "a" To "b"
            Debug.Print "range"
    End Select
End Sub
