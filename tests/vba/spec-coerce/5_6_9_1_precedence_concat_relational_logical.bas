' INV: vbal.5.6.9.1
' SOURCE: spec ms-vbal 5.6.9.1 Operator precedence and associativity, additive > concatenation > relational > Not > And > Or > Xor > Eqv > Imp
' EXPECT: a & 1 + 2=a3
' EXPECT: concat = =True
' EXPECT: Not 1 = 2=True
' EXPECT: Or/And=True
' EXPECT: And/Not=False
' EXPECT: Xor/Or=False
' EXPECT: Imp/Eqv=True
' EXPECT: rel/arith=True
Public Sub Main()
    Debug.Print "a & 1 + 2=" & CStr("a" & 1 + 2)
    Debug.Print "concat = =" & CStr("a" & "b" = "ab")
    Debug.Print "Not 1 = 2=" & CStr(Not 1 = 2)
    Debug.Print "Or/And=" & CStr(True Or False And False)
    Debug.Print "And/Not=" & CStr(Not True And False)
    Debug.Print "Xor/Or=" & CStr(True Xor True Or True)
    Debug.Print "Imp/Eqv=" & CStr(False Imp False Eqv False)
    Debug.Print "rel/arith=" & CStr(1 + 1 = 2)
End Sub
