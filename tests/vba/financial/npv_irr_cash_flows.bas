' INV: func.npv, func.irr
' SOURCE: derived the Learn Npv and IRR pages: Npv discounts the first value by one period and IRR is the rate at which the net present value of flows starting now is zero; the roots were found outside the interpreter
' EXPECT: 3.76
' EXPECT: 16.32
' EXPECT: 0.1307
' EXPECT: 0.0890
Public Sub Main()
    Dim a(2) As Double, b(3) As Double
    a(0) = -100: a(1) = 60: a(2) = 60
    b(0) = -1000: b(1) = 300: b(2) = 400: b(3) = 500
    Debug.Print Format(Npv(0.1, a), "0.00")
    Debug.Print Format(Npv(0.08, b), "0.00")
    Debug.Print Format(IRR(a), "0.0000")
    Debug.Print Format(IRR(b), "0.0000")
End Sub
