' INV: func.pv
' SOURCE: derived the Learn Pv page: present value of regular payments and a final value; the factors were evaluated outside the interpreter
' EXPECT: 772.17
' EXPECT: 810.78
' EXPECT: 68.44
Public Sub Main()
    Debug.Print Format(Pv(0.05, 10, -100), "0.00")
    Debug.Print Format(Pv(0.05, 10, -100, 0, 1), "0.00")
    Debug.Print Format(Pv(0.04, 5, -200, 1000), "0.00")
End Sub
