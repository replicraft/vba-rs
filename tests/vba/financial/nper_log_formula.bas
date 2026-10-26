' INV: func.nper
' SOURCE: derived the Learn NPer page: the number of periods follows from the logarithm of the payment ratio; the logarithms were evaluated outside the interpreter
' EXPECT: 10.59
' EXPECT: 25.80
Public Sub Main()
    Debug.Print Format(NPer(0.01, -100, 1000), "0.00")
    Debug.Print Format(NPer(0.02, -250, 5000), "0.00")
End Sub
