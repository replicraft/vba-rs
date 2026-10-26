' INV: vbal.6.1.2.7.1.5, func.iserror
' SOURCE: spec ms-vbal 6.1.2.7.1.5 IsError true only for Error data values
' EXPECT: cverr=True
' EXPECT: variant=True
' EXPECT: number=False
' EXPECT: string=False
' EXPECT: empty=False
Public Sub Main()
    Dim v As Variant
    v = CVErr(11)
    Debug.Print "cverr=" & CStr(IsError(CVErr(5)))
    Debug.Print "variant=" & CStr(IsError(v))
    Debug.Print "number=" & CStr(IsError(5))
    Debug.Print "string=" & CStr(IsError("#N/A"))
    Debug.Print "empty=" & CStr(IsError(Empty))
End Sub
