' INV: func.isempty, func.isnull, func.isobject, func.iserror
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(IsEmpty(Empty))
    Debug.Print CStr(IsEmpty(0))
    Debug.Print CStr(IsEmpty(""))
    Debug.Print CStr(IsNull(Null))
    Debug.Print CStr(IsNull(Empty))
    Debug.Print CStr(IsNull(""))
    Debug.Print CStr(IsObject(Nothing))
    Debug.Print CStr(IsObject(1))
    Debug.Print CStr(IsError(CVErr(3)))
    Debug.Print CStr(IsError(3))
End Sub
