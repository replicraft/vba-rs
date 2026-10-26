' INV: vbal.5.6.9.8, op.and, op.or, op.xor, op.eqv, op.imp
' SOURCE: spec ms-vbal 5.6.9.8 bitwise tables for And Or Xor Eqv Imp on integral values
' EXPECT: and=8 or=14 xor=6 eqv=-7 imp=-5
' EXPECT: and=0 or=-1 xor=-1 eqv=0 imp=0
' EXPECT: n=-6 n=-1 n=0
Public Sub Main()
    Dim a As Long, b As Long
    a = 12: b = 10
    Debug.Print "and=" & (a And b) & " or=" & (a Or b) & " xor=" & (a Xor b) & " eqv=" & (a Eqv b) & " imp=" & (a Imp b)
    a = 0: b = -1
    Debug.Print "and=" & (a And b) & " or=" & (a Or b) & " xor=" & (a Xor b) & " eqv=" & (a Eqv b) & " imp=" & (b Imp a)
    Debug.Print "n=" & (Not CLng(5)) & " n=" & (Not CLng(0)) & " n=" & (Not CLng(-1))
End Sub
