' INV: stmt.gosub-return, stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/gosubreturn-statement
' EXPECT:  42 
Function Twice(n As Long) As Long
    GoSub Calc
    Exit Function
Calc:
    Twice = n * 2
    Return
End Function
Public Sub Main()
    Debug.Print Twice(21)
End Sub
