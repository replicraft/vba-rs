' INV: stmt.gosub-return, stmt.function
' SOURCE: spec ms-vbal 5.4.2.14 GoSub statement, 5.4.2.15 Return statement
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
