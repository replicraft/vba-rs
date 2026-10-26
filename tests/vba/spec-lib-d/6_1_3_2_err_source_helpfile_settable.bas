' INV: vbal.6.1.3.2.2.2, vbal.6.1.3.2.2.3, vbal.6.1.3.2.2.6
' SOURCE: spec ms-vbal 6.1.3.2.2 Source, HelpFile, HelpContext, Description are read/write properties
' EXPECT: src=Proj.Cls
' EXPECT: hf=x.hlp
' EXPECT: hc=123
' EXPECT: ds=text
' EXPECT: t=String,String,Long
Public Sub Main()
    Err.Source = "Proj.Cls"
    Err.HelpFile = "x.hlp"
    Err.HelpContext = 123
    Err.Description = "text"
    Debug.Print "src=" & Err.Source
    Debug.Print "hf=" & Err.HelpFile
    Debug.Print "hc=" & CStr(Err.HelpContext)
    Debug.Print "ds=" & Err.Description
    Debug.Print "t=" & TypeName(Err.Source) & "," & TypeName(Err.HelpFile) & "," & TypeName(Err.HelpContext)
End Sub
