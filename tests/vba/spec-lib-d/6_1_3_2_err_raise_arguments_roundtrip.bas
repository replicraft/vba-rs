' INV: vbal.6.1.3.2.1.2
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Raise sets Number, Source, Description, HelpFile, HelpContext; vbObjectError offset
' EXPECT: num=5
' EXPECT: src=MySrc
' EXPECT: desc=MyDesc
' EXPECT: hf=c:\help.hlp
' EXPECT: hc=42
' EXPECT: hct=Long
' EXPECT: obj=True
' EXPECT: named=1051
' EXPECT: namedsrc=SomeClass
Public Sub Main()
    On Error Resume Next
    Err.Raise 5, "MySrc", "MyDesc", "c:\help.hlp", 42
    Debug.Print "num=" & CStr(Err.Number)
    Debug.Print "src=" & Err.Source
    Debug.Print "desc=" & Err.Description
    Debug.Print "hf=" & Err.HelpFile
    Debug.Print "hc=" & CStr(Err.HelpContext)
    Debug.Print "hct=" & TypeName(Err.HelpContext)
    Err.Clear
    Err.Raise vbObjectError + 513
    Debug.Print "obj=" & CStr(Err.Number = vbObjectError + 513)
    Err.Clear
    Err.Raise Number:=vbObjectError + 1051, Source:="SomeClass"
    Debug.Print "named=" & CStr(Err.Number - vbObjectError)
    Debug.Print "namedsrc=" & Err.Source
End Sub
