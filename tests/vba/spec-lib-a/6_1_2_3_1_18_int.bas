' INV: vbal.6.1.2.3.1.18, func.int
' SOURCE: spec ms-vbal 6.1.2.3.1.18 Int: floor, types preserved
' EXPECT: pos27=2
' EXPECT: neg27=-3
' EXPECT: neg25=-3
' EXPECT: pos05=0
' EXPECT: negIntegral=-4
' EXPECT: dblType=Double
' EXPECT: intType=Integer
' EXPECT: longType=Long
' EXPECT: longVal=70000
' EXPECT: byteType=Byte
' EXPECT: byteVal=5
' EXPECT: sng=-3
' EXPECT: sngType=Single
' EXPECT: cur=-3
' EXPECT: curType=Currency
' EXPECT: dec=2
' EXPECT: decType=Decimal
' EXPECT: str=-3
' EXPECT: strType=Double
' EXPECT: bool=-1
' EXPECT: boolType=Integer
' EXPECT: empty=0
' EXPECT: emptyType=Integer
' EXPECT: null=True
' EXPECT: date=-2
' EXPECT: dateType=Date
Public Sub Main()
    Dim e As Variant
    Dim b As Byte
    b = 5
    Debug.Print "pos27=" & CStr(Int(2.7))
    Debug.Print "neg27=" & CStr(Int(-2.7))
    Debug.Print "neg25=" & CStr(Int(-2.5))
    Debug.Print "pos05=" & CStr(Int(0.5))
    Debug.Print "negIntegral=" & CStr(Int(-4#))
    Debug.Print "dblType=" & TypeName(Int(2.7))
    Debug.Print "intType=" & TypeName(Int(5))
    Debug.Print "longType=" & TypeName(Int(70000))
    Debug.Print "longVal=" & CStr(Int(70000))
    Debug.Print "byteType=" & TypeName(Int(b))
    Debug.Print "byteVal=" & CStr(Int(b))
    Debug.Print "sng=" & CStr(Int(CSng(-2.5)))
    Debug.Print "sngType=" & TypeName(Int(CSng(-2.5)))
    Debug.Print "cur=" & CStr(Int(CCur(-2.5)))
    Debug.Print "curType=" & TypeName(Int(CCur(-2.5)))
    Debug.Print "dec=" & CStr(Int(CDec(2.7)))
    Debug.Print "decType=" & TypeName(Int(CDec(2.7)))
    Debug.Print "str=" & CStr(Int("-2.5"))
    Debug.Print "strType=" & TypeName(Int("-2.5"))
    Debug.Print "bool=" & CStr(Int(True))
    Debug.Print "boolType=" & TypeName(Int(True))
    Debug.Print "empty=" & CStr(Int(e))
    Debug.Print "emptyType=" & TypeName(Int(e))
    Debug.Print "null=" & CStr(IsNull(Int(Null)))
    Debug.Print "date=" & CStr(CDbl(Int(CDate(-1.5))))
    Debug.Print "dateType=" & TypeName(Int(CDate(-1.5)))
End Sub
