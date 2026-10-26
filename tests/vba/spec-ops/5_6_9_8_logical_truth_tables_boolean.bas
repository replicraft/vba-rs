' INV: vbal.5.6.9.8, op.and, op.or, op.xor, op.eqv, op.imp
' SOURCE: spec ms-vbal 5.6.9.8 single-bit truth tables over Boolean operands
' EXPECT: And FFFT
' EXPECT: Or FTTT
' EXPECT: Xor FTTF
' EXPECT: Eqv TFFT
' EXPECT: Imp TTFT
Private Function B(ByVal x As Boolean) As String
    If x Then B = "T" Else B = "F"
End Function
Public Sub Main()
    Dim f As Boolean, t As Boolean
    f = False: t = True
    Debug.Print "And " & B(f And f) & B(f And t) & B(t And f) & B(t And t)
    Debug.Print "Or " & B(f Or f) & B(f Or t) & B(t Or f) & B(t Or t)
    Debug.Print "Xor " & B(f Xor f) & B(f Xor t) & B(t Xor f) & B(t Xor t)
    Debug.Print "Eqv " & B(f Eqv f) & B(f Eqv t) & B(t Eqv f) & B(t Eqv t)
    Debug.Print "Imp " & B(f Imp f) & B(f Imp t) & B(t Imp f) & B(t Imp t)
End Sub
