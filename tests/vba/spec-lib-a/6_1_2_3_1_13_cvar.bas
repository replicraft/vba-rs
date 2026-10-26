' INV: vbal.6.1.2.3.1.13, func.cvar
' SOURCE: spec ms-vbal 6.1.2.3.1.13 CVar returns the argument value unchanged
' EXPECT: int=Integer
' EXPECT: dbl=Double
' EXPECT: str=String
' EXPECT: bool=Boolean
' EXPECT: byte=Byte
' EXPECT: val=abc
' EXPECT: null=True
' EXPECT: error=True
' EXPECT: errNum=Error 9
Public Sub Main()
    Dim b As Byte
    b = 3
    Debug.Print "int=" & TypeName(CVar(5))
    Debug.Print "dbl=" & TypeName(CVar(5.5))
    Debug.Print "str=" & TypeName(CVar("x"))
    Debug.Print "bool=" & TypeName(CVar(True))
    Debug.Print "byte=" & TypeName(CVar(b))
    Debug.Print "val=" & CStr(CVar("abc"))
    Debug.Print "null=" & CStr(IsNull(CVar(Null)))
    Debug.Print "error=" & CStr(IsError(CVar(CVErr(9))))
    Debug.Print "errNum=" & CStr(CVar(CVErr(9)))
End Sub
