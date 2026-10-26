' INV: func.irr
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 0.1307
Public Sub Main()
    Dim v(0 To 2) As Double
    v(0) = -100: v(1) = 60: v(2) = 60
    Debug.Print CStr(Round(IRR(v), 4))
End Sub
