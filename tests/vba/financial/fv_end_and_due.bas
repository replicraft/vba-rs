' INV: func.fv
' SOURCE: derived the Learn Fv page: future value of regular payments, an annuity due compounds one more period; the factors were evaluated outside the interpreter
' EXPECT: 1257.79
' EXPECT: 1320.68
' EXPECT: 1760.95
Public Sub Main()
    Debug.Print Format(Fv(0.05, 10, -100), "0.00")
    Debug.Print Format(Fv(0.05, 10, -100, 0, 1), "0.00")
    Debug.Print Format(Fv(0.01, 12, -50, -1000), "0.00")
End Sub
