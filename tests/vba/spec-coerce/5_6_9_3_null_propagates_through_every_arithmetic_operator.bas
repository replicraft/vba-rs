' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Null with any numeric, String, Date, Empty or Null operand gives Null
' EXPECT: add=True
' EXPECT: sub=True
' EXPECT: mul=True
' EXPECT: div=True
' EXPECT: idiv=True
' EXPECT: mod=True
' EXPECT: pow=True
' EXPECT: neg=True
' EXPECT: left=True
' EXPECT: string=True
' EXPECT: empty=True
' EXPECT: date=True
' EXPECT: nullnull=True
' EXPECT: type=Null
Public Sub Main()
    Dim n As Variant
    Dim e As Variant
    Dim d As Date
    n = Null
    d = 5
    Debug.Print "add=" & CStr(IsNull(n + 1))
    Debug.Print "sub=" & CStr(IsNull(n - 1))
    Debug.Print "mul=" & CStr(IsNull(n * 1))
    Debug.Print "div=" & CStr(IsNull(n / 1))
    Debug.Print "idiv=" & CStr(IsNull(n \ 1))
    Debug.Print "mod=" & CStr(IsNull(n Mod 1))
    Debug.Print "pow=" & CStr(IsNull(n ^ 1))
    Debug.Print "neg=" & CStr(IsNull(-n))
    Debug.Print "left=" & CStr(IsNull(1 + n))
    Debug.Print "string=" & CStr(IsNull(n + "3"))
    Debug.Print "empty=" & CStr(IsNull(n + e))
    Debug.Print "date=" & CStr(IsNull(n + d))
    Debug.Print "nullnull=" & CStr(IsNull(n + n))
    Debug.Print "type=" & TypeName(n * 2)
End Sub
