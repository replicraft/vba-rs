' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 the Is keyword before a comparison operator is optional
' EXPECT: eq
Public Sub Main()
    Select Case 5
        Case < 5
            Debug.Print "lt"
        Case = 5
            Debug.Print "eq"
    End Select
End Sub
