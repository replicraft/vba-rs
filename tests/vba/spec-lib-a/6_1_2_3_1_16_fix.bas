' INV: vbal.6.1.2.3.1.16, func.fix
' SOURCE: spec ms-vbal 6.1.2.3.1.16 Fix: types preserved, negative values truncate toward zero
' EXPECT: neg27=-2
' EXPECT: neg25=-2
' EXPECT: neg05=0
' EXPECT: negDblType=Double
' EXPECT: int=5
' EXPECT: intType=Integer
' EXPECT: longType=Long
' EXPECT: longVal=70000
' EXPECT: byteType=Byte
' EXPECT: byteVal=5
' EXPECT: sng=-2
' EXPECT: sngType=Single
' EXPECT: cur=-2
' EXPECT: curType=Currency
' EXPECT: dec=-2
' EXPECT: decType=Decimal
' EXPECT: str=-2
' EXPECT: strType=Double
' EXPECT: bool=-1
' EXPECT: boolType=Integer
' EXPECT: empty=0
' EXPECT: emptyType=Integer
' EXPECT: null=True
' EXPECT: date=-1
' EXPECT: dateType=Date
Public Sub Main()
    Dim e As Variant
    Dim b As Byte
    b = 5
    Debug.Print "neg27=" & CStr(Fix(-2.7))
    Debug.Print "neg25=" & CStr(Fix(-2.5))
    Debug.Print "neg05=" & CStr(Fix(-0.5))
    Debug.Print "negDblType=" & TypeName(Fix(-2.7))
    Debug.Print "int=" & CStr(Fix(5))
    Debug.Print "intType=" & TypeName(Fix(5))
    Debug.Print "longType=" & TypeName(Fix(70000))
    Debug.Print "longVal=" & CStr(Fix(70000))
    Debug.Print "byteType=" & TypeName(Fix(b))
    Debug.Print "byteVal=" & CStr(Fix(b))
    Debug.Print "sng=" & CStr(Fix(CSng(-2.5)))
    Debug.Print "sngType=" & TypeName(Fix(CSng(-2.5)))
    Debug.Print "cur=" & CStr(Fix(CCur(-2.7)))
    Debug.Print "curType=" & TypeName(Fix(CCur(-2.7)))
    Debug.Print "dec=" & CStr(Fix(CDec(-2.7)))
    Debug.Print "decType=" & TypeName(Fix(CDec(-2.7)))
    Debug.Print "str=" & CStr(Fix("-2.7"))
    Debug.Print "strType=" & TypeName(Fix("-2.7"))
    Debug.Print "bool=" & CStr(Fix(True))
    Debug.Print "boolType=" & TypeName(Fix(True))
    Debug.Print "empty=" & CStr(Fix(e))
    Debug.Print "emptyType=" & TypeName(Fix(e))
    Debug.Print "null=" & CStr(IsNull(Fix(Null)))
    Debug.Print "date=" & CStr(CDbl(Fix(CDate(-1.5))))
    Debug.Print "dateType=" & TypeName(Fix(CDate(-1.5)))
End Sub
