Shader "Custom/Shadow0"
{
  Properties
  {
    _LightPos ("Light Position", Vector) = (0,3,0)
    _SpherePos ("Sphere Position", Vector) = (0,1,0) // used for source of shadow
  }

  SubShader
  {
    Tags { "RenderType" = "Opaque" "RenderPipeline" = "UniversalPipeline" }
    Pass {
      HLSLPROGRAM
      #pragma vertex vert
      #pragma fragment frag
      #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

      struct Attributes {
        float4 positionOS : POSITION;
        float4 normalOS : NORMAL;
      };

      struct Varyings {
        float4 positionHCS : SV_POSITION;
        // float3 normalWS : TEXCOORD0; -> no longer needed
      };

      float4 _LightPos;
      float4 _SpherePos; // sphere variable declared in hlsl

      Varyings vert(Attributes IN) {
        Varyings OUT;

        // new
        float3 vertexWS = TransformObjectToWorld(IN.positionOS.xyz); // object space to world space
        float3 direction = vertexWS - _SpherePos.xyz; // from sphere to vertex
        float planeY = -0.5; // not synced with plane from hierarchy window
        float rayParameter = -(vertexWS.y - planeY) / direction.y; // project on plane y = 0
        float3 shadowPosition = vertexWS + rayParameter * direction;
        shadowPosition.y = planeY + 0.001;
        OUT.positionHCS = TransformObjectToHClip(shadowPosition);

        // OUT.positionHCS = TransformObjectToHClip(IN.positionOS.xyz); -> no need for calculating illumination
        // OUT.normalWS = TransformObjectToWorldNormal(IN.normalOS.xyz); -> replaced with shadow position code line above
        return OUT;
      } 

      half4 frag(Varyings IN) : SV_Target {
        // half4 color = half4(IN.normalWS * 0.5 + 0.5, 1.0); -> replaced with line below
        // return color; -> replaced with line below
        return half4(0.1, 0.1, 0.1, 1.0);
      }
      ENDHLSL
    }
  }
}