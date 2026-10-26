' INV: vbal.5.4.5.11, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 Get into a Variant reads the type descriptor and the value
' EXPECT: 5 2
' EXPECT: 7 3
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, w As Variant
    Open "/t/g.bin" For Binary As #1
    v = CInt(5)
    Put #1, , v
    v = CLng(7)
    Put #1, , v
    Seek #1, 1
    Get #1, , w
    Debug.Print CStr(w) & " " & CStr(VarType(w))
    Get #1, , w
    Debug.Print CStr(w) & " " & CStr(VarType(w))
    Close #1
End Sub
