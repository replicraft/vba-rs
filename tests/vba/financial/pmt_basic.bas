' INV: func.pmt
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 377.42
' EXPECT: 100
' EXPECT: 87.97
Public Sub Main()
    Debug.Print CStr(Round(Pmt(0.05 / 12, 60, -20000), 2))
    Debug.Print CStr(Round(Pmt(0, 10, -1000), 2))
    Debug.Print CStr(Round(Pmt(0.01, 12, -1000, 0, 1), 2))
End Sub
