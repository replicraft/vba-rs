' INV: func.ipmt, func.ppmt, func.pmt
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 0
' EXPECT: 10
Public Sub Main()
    Debug.Print CStr(Round(IPmt(0.01, 1, 12, -1000) + PPmt(0.01, 1, 12, -1000) - Pmt(0.01, 12, -1000), 6))
    Debug.Print CStr(Round(IPmt(0.01, 1, 12, -1000), 2))
End Sub
