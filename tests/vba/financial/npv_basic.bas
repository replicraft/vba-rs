' INV: func.npv
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 248.69
Public Sub Main()
    Dim v(0 To 2) As Double
    v(0) = 100: v(1) = 100: v(2) = 100
    Debug.Print CStr(Round(NPV(0.1, v), 2))
End Sub
