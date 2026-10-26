' INV: op.or
' SOURCE: derived the Learn operator pages: the logical operators work bit by bit on integral operands
' EXPECT: 8 14 6
' EXPECT: -13 -7 -5
' EXPECT: -1 True
Public Sub Main()
    Debug.Print CStr(12 And 10) & " " & CStr(12 Or 10) & " " & CStr(12 Xor 10)
    Debug.Print CStr(Not 12) & " " & CStr(12 Eqv 10) & " " & CStr(12 Imp 10)
    Debug.Print CStr(True And -1) & " " & CStr(Not 0 = -1)
End Sub
