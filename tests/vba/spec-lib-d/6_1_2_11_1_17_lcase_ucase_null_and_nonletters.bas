' INV: vbal.6.1.2.11.1.17, func.lcase, func.ucase
' SOURCE: spec ms-vbal 6.1.2.11.1.17 LCase/UCase convert letters only, Null gives Null
' EXPECT: l=abc123!_
' EXPECT: u=ABC123!_
' EXPECT: ln=True
' EXPECT: un=True
' EXPECT: lt=Null
' EXPECT: d=abc
' EXPECT: e=ABC
Public Sub Main()
    Debug.Print "l=" & LCase("AbC123!_")
    Debug.Print "u=" & UCase("aBc123!_")
    Debug.Print "ln=" & CStr(IsNull(LCase(Null)))
    Debug.Print "un=" & CStr(IsNull(UCase(Null)))
    Debug.Print "lt=" & TypeName(LCase(Null))
    Debug.Print "d=" & LCase$("ABC")
    Debug.Print "e=" & UCase$("abc")
End Sub
